# Review: KISS: nova_gameplay HUD - combat readout widgets

- TASK: 20260731-170329
- BRANCH: refactor/kiss-hud-combat-readout

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

No BLOCKER or MAJOR. Every finding is MINOR or NIT and was fixed or answered
within the round; the reviewer re-checked each fix and confirmed all five,
found no new defect, and accepted the one decline. The in-session pass
re-derived R1.1, R1.2 and R1.4 from the tree before accepting them, and
re-ran `cargo fmt --check` (exit 0) and `cargo check --workspace
--all-targets` (exit 0) after the fixes.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/hud/item_highlights.rs:8 - the
  rewrap left a module-doc line starting with `- `, which CommonMark parses
  as a list item interrupting the paragraph, so rustdoc renders a stray
  bullet mid-sentence: a real rendering change inside a "no behavior change"
  pass. Reflow so no line begins with a dash.
  - Response: confirmed by reading lines 5-10 - the dash had moved to
    column 5 of the next line. Reflowed the paragraph; the dash is now
    line-final on the preceding line.
- [x] R1.2 (MINOR) tasks/20260731-170329/NOTES.md:29 - "Every file is over
  half tests" is false for 7 of the 12, and the `824` given as
  screen_indicator's prod count is its *test* count (`mod tests` opens at
  606, so prod is 604). Replace with the measured split.
  - Response: confirmed - `#[cfg(test)]` at screen_indicator.rs:605. Both
    numbers were wrong and the summary sentence overstated the case. NOTES
    now carries a measured prod/tests column per file and claims only what
    those numbers support. Reviewer re-measured all 24 figures.
- [x] R1.3 (MINOR) crates/nova_gameplay/src/hud/ammo_readout.rs:43 - the
  public module doc intra-doc-links `[sync_ammo_gate]`, a private fn; NOTES
  records the warning but no task was filed, though Step 5 requires one.
  Drop the brackets or file it.
  - Response: pre-existing on master, so out of this diff and not fixed
    here, but the reviewer is right that it was recorded and not filed.
    Added to backlog 20260731-205553 with the drop-the-brackets remedy named
    (making the fn public would be the wrong direction).
- [x] R1.4 (MINOR) crates/nova_gameplay/src/hud/lock_crosshairs.rs:517 -
  four provenance clauses of the categories NOTES claims were cut survive:
  two `(playtest 2026-07-13)` (here and target_inset.rs:1201) and two
  `(review Rn.n)` (torpedo_target.rs:1150, 1155). Strip or record as
  deliberate keeps.
  - Response: confirmed at all four sites and stripped, keeping the
    constraint prose in each. Root cause worth naming: these carry no tatr
    ID, so DoD 3's grep can never see them - the proof command was narrower
    than the claim it gated. NOTES now says so.
- [ ] R1.5 (NIT) crates/nova_gameplay/src/hud/allegiance_markers.rs:16 -
  several rewrapped comments leave a short orphan line where text was
  deleted mid-line. Reflow those paragraphs to the file's ~80-col fill.
  - Response: declined. Every line is inside the fill and reads correctly;
    reflowing would touch ~15 more comment blocks for appearance alone,
    enlarging a diff whose whole value is being auditable as no-change.
    R1.1 was reflowed because ragged wrapping there changed the RENDER, not
    the shape. Reviewer accepted the distinction.
- [x] R1.6 (NIT) crates/nova_gameplay/src/hud/torpedo_target.rs:86 - a
  seventh deleted pointer, a bare `(DECISION.md)`, is missing from NOTES's
  inventory of six, and a matching record does exist at
  `tasks/20260730-123009/DECISION.md`. Add it, noting it was unresolvable as
  written.
  - Response: added. NOTES also now says the six `docs/spikes/*.md` links
    were dead **as paths** while the spike content survives as
    `tasks/<id>/SPIKE.md` - the reviewer's correction, and the more honest
    claim.

Verified independently in-session: DoD 3 grep returns zero over the 12
scoped files; DoD 4 holds (largest 1428 < 1500); `git diff -U0 master...HEAD`
over `crates/` still shows exactly 3 non-comment changed lines, the test
assertion strings NOTES names. The reviewer separately confirmed the fix
diff is comment-only with insertions equal to deletions per file, so no
executable line shifted.

Pending user checks (do not block APPROVE):

- DoD 5 (`test:` existing tests in this area still pass) - not run locally;
  the link step exhausts this machine's RAM. CI runs the suite on the PR.
- DoD 6 (`manual:` owner skims the diff and agrees no behavior changed).
